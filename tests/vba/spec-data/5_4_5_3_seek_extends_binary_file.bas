' INV: vbal.5.4.5.3, stmt.seek
' SOURCE: spec ms-vbal 5.4.5.3 Seek beyond the end extends the file to the new position
' XFAIL: wrong pending
' EXPECT: 10
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Open "/t/s.bin" For Binary As #1
    Seek #1, 10
    Debug.Print CStr(LOF(1))
    Close #1
End Sub
