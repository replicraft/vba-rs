' INV: vbal.5.4.5.1, stmt.get, stmt.put
' SOURCE: spec ms-vbal 5.4.5.1 Get and Put are valid on Binary with default access
' EXPECT: 12345
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Dim n As Long, m As Long
    n = 12345
    Open "/t/m.bin" For Binary As #1
    Put #1, 1, n
    Get #1, 1, m
    Close #1
    Debug.Print CStr(m)
End Sub
