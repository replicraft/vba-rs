Attribute VB_Name = "OracleCalibrate"
Option Explicit
'
' OracleCalibrate - checks the central assumption of the oracle capture.
'
' STATUS: UNTESTED. Never compiled or run (written without Office).
'
' The oracle run records the output of `Debug.Print` by rewriting it into
' `Print #255, ...`. That is only valid if both produce the same text. This
' module prints the same items both ways so a person can compare them ONCE per
' host version:
'
'   1. Import this module into a scratch workbook/database (Alt+F11, File >
'      Import File).
'   2. In the Immediate window (Ctrl+G) run:   CalibrateByEye "C:\oracle\calibrate.txt"
'   3. Click in the Immediate window, Ctrl+A, Ctrl+C; paste into
'      C:\oracle\immediate.txt (Notepad) and save as ANSI.
'   4. In PowerShell:  Compare-Object (Get-Content C:\oracle\immediate.txt) (Get-Content C:\oracle\calibrate.txt)
'      The two files must be identical; ANY difference
'      means the shim is not faithful for that item: record it in the results
'      you send back and do not upgrade provenance from that environment.
'
' Every line shows the item between "[" and "]" so leading and trailing spaces
' (the sign column and the trailing space after numbers) are visible.

Public Sub CalibrateByEye(ByVal logPath As String)
    Open logPath For Output As #255
    Debug.Print "=== 1 integer": Print #255, "=== 1 integer"
    Debug.Print "["; 5; "]": Print #255, "["; 5; "]"
    Debug.Print "=== 2 negative": Print #255, "=== 2 negative"
    Debug.Print "["; -5; "]": Print #255, "["; -5; "]"
    Debug.Print "=== 3 long zero": Print #255, "=== 3 long zero"
    Debug.Print "["; 0; "]": Print #255, "["; 0; "]"
    Debug.Print "=== 4 double": Print #255, "=== 4 double"
    Debug.Print "["; 1.5; "]": Print #255, "["; 1.5; "]"
    Debug.Print "=== 5 double negative": Print #255, "=== 5 double negative"
    Debug.Print "["; -1.5; "]": Print #255, "["; -1.5; "]"
    Debug.Print "=== 6 single": Print #255, "=== 6 single"
    Debug.Print "["; CSng(0.1); "]": Print #255, "["; CSng(0.1); "]"
    Debug.Print "=== 7 large double": Print #255, "=== 7 large double"
    Debug.Print "["; 1E+20; "]": Print #255, "["; 1E+20; "]"
    Debug.Print "=== 8 small double": Print #255, "=== 8 small double"
    Debug.Print "["; 1.5E-10; "]": Print #255, "["; 1.5E-10; "]"
    Debug.Print "=== 9 currency": Print #255, "=== 9 currency"
    Debug.Print "["; CCur(1.25); "]": Print #255, "["; CCur(1.25); "]"
    Debug.Print "=== 10 boolean true": Print #255, "=== 10 boolean true"
    Debug.Print "["; True; "]": Print #255, "["; True; "]"
    Debug.Print "=== 11 boolean false": Print #255, "=== 11 boolean false"
    Debug.Print "["; False; "]": Print #255, "["; False; "]"
    Debug.Print "=== 12 date": Print #255, "=== 12 date"
    Debug.Print "["; DateSerial(2020, 1, 2); "]": Print #255, "["; DateSerial(2020, 1, 2); "]"
    Debug.Print "=== 13 date and time": Print #255, "=== 13 date and time"
    Debug.Print "["; DateSerial(2020, 1, 2) + TimeSerial(3, 4, 5); "]": Print #255, "["; DateSerial(2020, 1, 2) + TimeSerial(3, 4, 5); "]"
    Debug.Print "=== 14 time only": Print #255, "=== 14 time only"
    Debug.Print "["; TimeSerial(3, 4, 5); "]": Print #255, "["; TimeSerial(3, 4, 5); "]"
    Debug.Print "=== 15 string": Print #255, "=== 15 string"
    Debug.Print "["; "abc"; "]": Print #255, "["; "abc"; "]"
    Debug.Print "=== 16 empty string": Print #255, "=== 16 empty string"
    Debug.Print "["; ""; "]": Print #255, "["; ""; "]"
    Debug.Print "=== 17 null": Print #255, "=== 17 null"
    Debug.Print "["; Null; "]": Print #255, "["; Null; "]"
    Debug.Print "=== 18 empty": Print #255, "=== 18 empty"
    Debug.Print "["; Empty; "]": Print #255, "["; Empty; "]"
    Debug.Print "=== 19 error value": Print #255, "=== 19 error value"
    Debug.Print "["; CVErr(7); "]": Print #255, "["; CVErr(7); "]"
    Debug.Print "=== 20 byte": Print #255, "=== 20 byte"
    Debug.Print "["; CByte(7); "]": Print #255, "["; CByte(7); "]"
    Debug.Print "=== 21 semicolon items": Print #255, "=== 21 semicolon items"
    Debug.Print "a"; "b"; 1; 2: Print #255, "a"; "b"; 1; 2
    Debug.Print "=== 22 comma zones": Print #255, "=== 22 comma zones"
    Debug.Print "a", "b", "c": Print #255, "a", "b", "c"
    Debug.Print "=== 23 comma zones with numbers": Print #255, "=== 23 comma zones with numbers"
    Debug.Print 1, 22, 333: Print #255, 1, 22, 333
    Debug.Print "=== 24 spc": Print #255, "=== 24 spc"
    Debug.Print "a"; Spc(3); "b": Print #255, "a"; Spc(3); "b"
    Debug.Print "=== 25 tab": Print #255, "=== 25 tab"
    Debug.Print "a"; Tab(10); "b": Print #255, "a"; Tab(10); "b"
    Debug.Print "=== 26 tab without argument": Print #255, "=== 26 tab without argument"
    Debug.Print "a"; Tab; "b": Print #255, "a"; Tab; "b"
    Debug.Print "=== 27 tab behind position": Print #255, "=== 27 tab behind position"
    Debug.Print "abcdefghijkl"; Tab(5); "b": Print #255, "abcdefghijkl"; Tab(5); "b"
    Debug.Print "=== 28 comma then tab": Print #255, "=== 28 comma then tab"
    Debug.Print "a", Tab(40); "z": Print #255, "a", Tab(40); "z"
    Debug.Print "=== 29 trailing semicolon": Print #255, "=== 29 trailing semicolon"
    Debug.Print "a";: Print #255, "a";
    Debug.Print "b": Print #255, "b"
    Debug.Print "=== 30 trailing comma": Print #255, "=== 30 trailing comma"
    Debug.Print "a",: Print #255, "a",
    Debug.Print "b": Print #255, "b"
    Debug.Print "=== 31 blank line": Print #255, "=== 31 blank line"
    Debug.Print: Print #255, ""
    Close #255
End Sub
