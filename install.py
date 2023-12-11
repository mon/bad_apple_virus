import os
import shutil

print("Welcome to Bad_Apple_Virus Installer.")

isCargoInstalled = shutil.which("cargo")

if (isCargoInstalled != ""):
    print("============================================")
    print("= Cargo and Rust are Installed... Skipping =")
    print("============================================")
else:
    print("===================================================================")
    print("= Cargo and Rust are not installed... Starting the Rust installer =.")
    print("===================================================================")
    os.system(".\\assets\\dependencies\\rustup-init.exe")


print("Building Bad Apple!!")
os.system("cargo build --release")
print("===========================<>=========================")
print("Bad apple installed ! Check the target/release folder !")
print("Do you want to launch Bad Apple ? (Y, N)")

run = input()

if (run=="N" or run == "n"):
    exit()
else:
    print("Launching Bad Apple!!")
    os.system(".\\target\\release\\bad_apple.exe")
    exit()