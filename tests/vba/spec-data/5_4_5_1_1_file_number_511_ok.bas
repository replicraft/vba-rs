' INV: vbal.5.4.5.1.1, stmt.open
' SOURCE: spec ms-vbal 5.4.5.1.1 file numbers 1 and 511 are valid
' EXPECT: ok
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Open "/t/f1" For Output As #511
    Open "/t/f2" For Output As #1
    Close #511
    Close #1
    Debug.Print "ok"
End Sub
