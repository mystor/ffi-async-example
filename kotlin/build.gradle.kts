plugins {
    kotlin("jvm") version "2.2.21"
    application
}

java {
    sourceCompatibility = JavaVersion.VERSION_24
    targetCompatibility = JavaVersion.VERSION_24
}

kotlin {
    compilerOptions {
        jvmTarget.set(org.jetbrains.kotlin.gradle.dsl.JvmTarget.JVM_24)
    }
}

dependencies {
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-core:1.10.2")
}

application {
    mainClass.set("KotlinBinding")
    applicationDefaultJvmArgs = listOf("-Djava.library.path=../rust-lib/target/debug")
}
