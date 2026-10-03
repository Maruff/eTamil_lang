@echo off
call "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat" >nul 2>&1
cl /nologo /O2 /Fe:bin\tax_int_c.exe tax_int.c >nul
cl /nologo /O2 /Fe:bin\tax_double_c.exe tax_double.c >nul
cl /nologo /O2 /Fe:bin\empty_c.exe empty.c >nul
rem C++ at the same /O2, so the pair differs by language and nothing else.
cl /nologo /O2 /EHsc /Fe:bin\tax_int_cpp.exe tax_int.cpp >nul
cl /nologo /O2 /EHsc /Fe:bin\tax_double_cpp.exe tax_double.cpp >nul
cl /nologo /O2 /EHsc /Fe:bin\empty_cpp.exe empty.cpp >nul
del *.obj 2>nul
echo C and C++ build done
