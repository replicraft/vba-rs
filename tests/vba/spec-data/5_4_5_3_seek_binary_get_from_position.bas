' INV: vbal.5.4.5.3, stmt.seek, stmt.get
' SOURCE: spec ms-vbal 5.4.5.3 Seek sets the file-pointer-position for the next operation
' EXPECT: CD
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Dim w As String, r As String * 2
    w = "ABCDEF"
    Open "/t/s.bin" For Binary As #1
    Put #1, , w
    Seek #1, 3
    Get #1, , r
    Close #1
    Debug.Print r
End Sub
