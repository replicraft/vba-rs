' INV: stmt.put, stmt.get
' SOURCE: derived the Learn Put and Get statement pages: the descriptor holds one entry per dimension; the data follows in storage order
' EXPECT: 2 1 0 1 len=26
' EXPECT: 4 1 3 2
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Dim a() As Integer, b() As Integer
    ReDim a(1 To 2, 0 To 1)
    a(1, 0) = 1: a(2, 0) = 2: a(1, 1) = 3: a(2, 1) = 4
    Open "/t/g.bin" For Binary As #1
    Put #1, , a
    Dim s As String
    s = "len=" & CStr(LOF(1))
    Seek #1, 1
    Get #1, , b
    Close #1
    Debug.Print CStr(UBound(b, 1)) & " " & CStr(LBound(b, 1)) & " " & CStr(LBound(b, 2)) & " " & CStr(UBound(b, 2)) & " " & s
    Debug.Print CStr(b(2, 1)) & " " & CStr(b(1, 0)) & " " & CStr(b(1, 1)) & " " & CStr(b(2, 0))
End Sub
