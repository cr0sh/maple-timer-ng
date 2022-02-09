# escape=`
FROM mcr.microsoft.com/dotnet/framework/sdk:4.8-windowsservercore-20H2

WORKDIR C:\\build
RUN `
    # Download the Build Tools bootstrapper.
    curl -SL --output vs_buildtools.exe https://aka.ms/vs/17/release/vs_buildtools.exe `
    `
    # Install Build Tools with the Microsoft.VisualStudio.Workload.AzureBuildTools workload, excluding workloads and components with known issues.
    && (start /w vs_buildtools.exe --quiet --wait --norestart --nocache modify `
    --installPath "%ProgramFiles(x86)%\Microsoft Visual Studio\2022\BuildTools" `
    --add Microsoft.VisualStudio.Workload.VCTools `
    --add Microsoft.VisualStudio.Component.CoreBuildTools `
    --add Microsoft.VisualStudio.Component.VC.Tools.x86.x64 `
    --add Microsoft.VisualStudio.Component.VC.CoreBuildTools `
    --add Microsoft.VisualStudio.Component.Windows10SDK.19041 `
    || IF "%ERRORLEVEL%"=="3010" EXIT 0) `
    `
    # Cleanup
    && del /q vs_buildtools.exe

RUN `
    powershell -Command "(Invoke-WebRequest -OutFile rustup-init.exe https://win.rustup.rs/x86_64) `
    ; (./rustup-init.exe --default-toolchain nightly-2022-01-06 -y --profile minimal) `
    ; rm ./rustup-init.exe"

RUN mkdir C:\\mount

COPY build_isolated.ps1 .

ENTRYPOINT [ "powershell", "-Command", "./build_isolated.ps1" ]