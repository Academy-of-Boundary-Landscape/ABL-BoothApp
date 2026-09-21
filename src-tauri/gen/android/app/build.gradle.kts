import java.util.Properties
import java.io.FileInputStream
import com.android.build.api.variant.impl.VariantOutputImpl

plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
    id("rust")
}

val tauriProperties = Properties().apply {
    val propFile = file("tauri.properties")
    if (propFile.exists()) {
        propFile.inputStream().use { load(it) }
    }
}

android {
    compileSdk = 36
    namespace = "com.abl.BoothKernel"

    defaultConfig {
        manifestPlaceholders["usesCleartextTraffic"] = "false"
        applicationId = "com.abl.BoothKernel"
        minSdk = 24
        targetSdk = 36
        versionCode = tauriProperties.getProperty("tauri.android.versionCode", "1").toInt()
        versionName = tauriProperties.getProperty("tauri.android.versionName", "1.0")

        // 真机发布当前只面向 arm64-v8a，避免继续暗示多 ABI 构建。
        ndk {
            abiFilters.add("arm64-v8a")
        }
    }

    signingConfigs {
        create("release") {
            val keystorePropertiesFile = rootProject.file("keystore.properties")
            val keystoreProperties = Properties()

            if (keystorePropertiesFile.exists()) {
                keystoreProperties.load(FileInputStream(keystorePropertiesFile))
            } else {
                throw GradleException("keystore.properties not found at: ${keystorePropertiesFile.absolutePath}")
            }

            keyAlias = keystoreProperties["keyAlias"] as String
            keyPassword = keystoreProperties["keyPassword"] as String
            storeFile = file(keystoreProperties["storeFile"] as String)
            storePassword = keystoreProperties["storePassword"] as String
        }
    }

    buildTypes {
        getByName("debug") {
            manifestPlaceholders["usesCleartextTraffic"] = "true"
            isDebuggable = true
            isJniDebuggable = true
            isMinifyEnabled = false
            packaging {
                jniLibs.keepDebugSymbols.add("*/arm64-v8a/*.so")
            }
        }

        getByName("release") {
            isMinifyEnabled = true
            signingConfig = signingConfigs.getByName("release")
            proguardFiles(
                *fileTree(".") { include("**/*.pro") }
                    .plus(getDefaultProguardFile("proguard-android-optimize.txt"))
                    .toList()
                    .toTypedArray()
            )
        }
    }

    kotlinOptions {
        jvmTarget = "1.8"
    }

    buildFeatures {
        buildConfig = true
    }
}

/**
 * APK 输出重命名：
 * - 默认读取 gradle.properties 的 artifactName（没有则用 BoothKernel）
 * - 版本号用 android.defaultConfig.versionName
 * - 当前项目实际只发布 arm64 APK，因此把 universal 变体名也重写成 arm64，减少误导
 */
androidComponents {
    onVariants(selector().all()) { variant ->
        val appName = (project.findProperty("artifactName") as String?)?.trim()
            ?.takeIf { it.isNotEmpty() }
            ?: "BoothKernel"

        val version = android.defaultConfig.versionName?.trim()
            ?.takeIf { it.isNotEmpty() }
            ?: "0.0.0"

        val variantSlug = variant.name
            .replace("universal", "arm64")
            .replace("Release", "-release")
            .replace("Debug", "-debug")
            .lowercase()

        variant.outputs.forEach { output ->
            (output as VariantOutputImpl).outputFileName =
                "${appName}-${version}-${variantSlug}.apk"
        }
    }
}

rust {
    rootDirRel = "../../../"
}

dependencies {
    implementation("androidx.webkit:webkit:1.14.0")
    implementation("androidx.appcompat:appcompat:1.7.1")
    implementation("androidx.activity:activity-ktx:1.10.1")
    implementation("com.google.android.material:material:1.12.0")

    testImplementation("junit:junit:4.13.2")
    androidTestImplementation("androidx.test.ext:junit:1.1.4")
    androidTestImplementation("androidx.test.espresso:espresso-core:3.5.0")
}

apply(from = "tauri.build.gradle.kts")
