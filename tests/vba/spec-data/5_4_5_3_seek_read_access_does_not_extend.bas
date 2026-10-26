' INV: vbal.5.4.5.3, stmt.seek
' SOURCE: spec ms-vbal 5.4.5.3 extension does not occur for files opened with Read access
' EXPECT: 4
' EXPECT: 4
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Dim n As Long
    n = 1
    Open "/t/s.bin" For Binary As #1
    Put #1, 1, n
    Close #1
    Open "/t/s.bin" For Binary Access Read As #1
    Debug.Print CStr(LOF(1))
    Seek #1, 100
    Debug.Print CStr(LOF(1))
    Close #1
End Sub
