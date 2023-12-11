import os
from urllib.request import urlretrieve
import subprocess

print("Welcome to Bad_Apple_Virus Installer.")
print("Please verify that Rust and Cargo are installed.")
print("Are rust and cargo installed ? (Y, N) : ")
install = input()

if (install=="N"):
    print("Install Rust then launch this script again.")
    exit()
else:
    print("Building Bad Apple!!")
    os.system("cargo build --release")
print("===========================<>=========================")
print("Bad apple installed ! Check the target/release folder !")
print("Do you want to launch Bad Apple ? (Y, N)")

run = input()

if (run=="N"):
    exit()
else:
    print("Launching Bad Apple!!")
    os.system(".\\target\\release\\bad_apple.exe")
    exit()