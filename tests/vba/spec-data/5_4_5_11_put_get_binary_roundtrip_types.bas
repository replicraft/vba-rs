' INV: vbal.5.4.5.11, stmt.put, stmt.get
' SOURCE: spec ms-vbal 5.4.5.11 values written in sequence are read back in sequence
' EXPECT: 258 70000 1.5 True
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Dim i As Integer, l As Long, d As Double, b As Boolean
    Dim i2 As Integer, l2 As Long, d2 As Double, b2 As Boolean
    i = 258: l = 70000: d = 1.5: b = True
    Open "/t/g.bin" For Binary As #1
    Put #1, , i
    Put #1, , l
    Put #1, , d
    Put #1, , b
    Seek #1, 1
    Get #1, , i2
    Get #1, , l2
    Get #1, , d2
    Get #1, , b2
    Close #1
    Debug.Print CStr(i2) & " " & CStr(l2) & " " & CStr(d2) & " " & CStr(b2)
End Sub
