import java.util.Properties
import java.io.FileInputStream
import com.android.build.api.variant.impl.VariantOutputImpl

plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
    id("rust")
}

// release 签名材料。文件不入库，迁移机器时必须手动带过来（换签名 = 老用户必须卸载重装）。
val keystorePropertiesFile = rootProject.file("keystore.properties")
val hasReleaseKeystore = keystorePropertiesFile.exists()
val keystoreProperties = Properties().apply {
    if (hasReleaseKeystore) {
        FileInputStream(keystorePropertiesFile).use { load(it) }
    }
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
        // keystore.properties 不入库（见仓库根 .gitignore）。缺失时不在 configure 阶段
        // 直接抛异常——否则连 debug 构建和全新 clone 都跑不起来。真正打 release 包时
        // 由下面的 assemble*Release 守卫失败，并把该补什么讲清楚。
        if (hasReleaseKeystore) {
            create("release") {
                keyAlias = keystoreProperties["keyAlias"] as String
                keyPassword = keystoreProperties["keyPassword"] as String
                storeFile = file(keystoreProperties["storeFile"] as String)
                storePassword = keystoreProperties["storePassword"] as String
            }
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
            signingConfig = signingConfigs.findByName("release")
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

// 没有 keystore.properties 时，release 产物会是未签名的 APK/AAB——那种包装不上、
// 也覆盖不了线上版本。与其悄悄产出废包，不如在构建入口直接失败。
if (!hasReleaseKeystore) {
    tasks.matching {
        (it.name.startsWith("assemble") || it.name.startsWith("bundle")) && it.name.endsWith("Release")
    }.configureEach {
        doFirst {
            throw GradleException(
                """
                |缺少 release 签名材料：${keystorePropertiesFile.absolutePath}
                |
                |新建该文件，内容为：
                |  storeFile=/abs/path/to/boothkernel-upload.keystore
                |  storePassword=<store 密码>
                |  keyAlias=<alias>
                |  keyPassword=<key 密码>
                |
                |必须是签过历史版本（v1.0.0 起）的那把 keystore，换密钥会导致老用户无法覆盖安装。
                |该文件已在 .gitignore，不要入库。
                """.trimMargin()
            )
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
