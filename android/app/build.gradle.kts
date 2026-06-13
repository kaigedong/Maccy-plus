plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
}

android {
    namespace = "com.kaigedong.maccy"
    compileSdk = 35

    defaultConfig {
        applicationId = "com.kaigedong.maccy"
        minSdk = 26
        targetSdk = 35
        versionCode = 1
        versionName = "1.0"
    }

    buildTypes {
        release {
            isMinifyEnabled = false
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    kotlinOptions {
        jvmTarget = "17"
    }

    buildFeatures {
        compose = true
    }

    composeOptions {
        kotlinCompilerExtensionVersion = "1.5.15"
    }

    sourceSets["main"].jniLibs.srcDirs("src/main/jniLibs")
}

dependencies {
    // Compose
    implementation(platform("androidx.compose:compose-bom:2024.12.01"))
    implementation("androidx.compose.ui:ui")
    implementation("androidx.compose.ui:ui-graphics")
    implementation("androidx.compose.ui:ui-tooling-preview")
    implementation("androidx.compose.material3:material3")
    implementation("androidx.compose.material:material-icons-extended")

    // AndroidX
    implementation("androidx.core:core-ktx:1.15.0")
    implementation("androidx.lifecycle:lifecycle-runtime-ktx:2.8.7")
    implementation("androidx.lifecycle:lifecycle-viewmodel-compose:2.8.7")
    implementation("androidx.activity:activity-compose:1.9.3")

    debugImplementation("androidx.compose.ui:ui-tooling")
}

// Build Rust and generate Kotlin bindings before compiling
tasks.register<Exec>("buildRustCore") {
    commandLine(
        "cargo", "build", "--release",
        "--target", "aarch64-linux-android",
        "--package", "maccy-core",
        "--manifest-path", file("../../Cargo.toml").absolutePath
    )
    onlyIf { !file("../../target/aarch64-linux-android/release/libmaccy_core.so").exists() }
}

tasks.register<Exec>("generateKotlinBindings") {
    dependsOn("buildRustCore")
    commandLine(
        "cargo", "run", "--release",
        "--bin", "uniffi-bindgen",
        "--package", "maccy-core",
        "generate",
        "--library", file("../../target/aarchy64-linux-android/release/libmaccy_core.so").absolutePath,
        "--language", "kotlin",
        "--out-dir", file("src/main/java").absolutePath
    )
}

tasks.named("preBuild") {
    dependsOn("generateKotlinBindings")
}
