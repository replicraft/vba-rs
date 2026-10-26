' INV: vbal.5.4.5.2, stmt.reset
' SOURCE: spec ms-vbal 5.4.5.2 Reset is a close statement for all files
' EXPECT: reset
' ERROR: 52
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Open "/t/a" For Output As #1
    Open "/t/b" For Output As #2
    Reset
    Debug.Print "reset"
    Print #1, "x"
End Sub
