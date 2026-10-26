' INV: vbal.5.4.5.1, stmt.put
' SOURCE: spec ms-vbal 5.4.5.1 Put # needs write access on Binary
' EXPECT: err=True
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Dim n As Long
    Open "/t/m.bin" For Binary As #1
    Put #1, 1, n
    Close #1
    Open "/t/m.bin" For Binary Access Read As #1
    On Error Resume Next
    Put #1, 1, n
    Debug.Print "err=" & CStr(Err.Number <> 0)
End Sub
