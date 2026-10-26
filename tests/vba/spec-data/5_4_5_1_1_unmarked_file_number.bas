' INV: vbal.5.4.5.1.1, stmt.open
' SOURCE: spec ms-vbal 5.4.5.1.1 file number without a number sign
' EXPECT: ok
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Open "/t/f" For Output As 2
    Print #2, "x"
    Close 2
    Debug.Print "ok"
End Sub
