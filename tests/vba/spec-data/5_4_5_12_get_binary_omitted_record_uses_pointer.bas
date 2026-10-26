' INV: vbal.5.4.5.12, stmt.get
' SOURCE: spec ms-vbal 5.4.5.12 omitted record number continues from the current file pointer
' EXPECT: 11 22
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Dim a As Long, b As Long
    a = 11: b = 22
    Open "/t/g.bin" For Binary As #1
    Put #1, , a
    Put #1, , b
    Seek #1, 1
    Get #1, , a
    Get #1, , b
    Close #1
    Debug.Print CStr(a) & " " & CStr(b)
End Sub
