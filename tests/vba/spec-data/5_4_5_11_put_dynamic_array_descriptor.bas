' INV: stmt.put
' SOURCE: derived the Learn Put and Get statement pages: a dynamic array is written as a descriptor of 2 + 8 * dimensions bytes followed by the data; the order of the two fields of a dimension is not documented, so count then lower bound is assumed
' EXPECT: len=14 dims=1 count=2 lower=2 data=1,2
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Dim a() As Integer
    ReDim a(2 To 3)
    a(2) = 1
    a(3) = 2
    Open "/t/g.bin" For Binary As #1
    Put #1, , a
    Dim n As Integer, c As Long, l As Long, x As Integer, y As Integer
    Dim s As String
    s = "len=" & CStr(LOF(1))
    Seek #1, 1
    Get #1, , n
    Get #1, , c
    Get #1, , l
    Get #1, , x
    Get #1, , y
    Close #1
    Debug.Print s & " dims=" & CStr(n) & " count=" & CStr(c) & " lower=" & CStr(l) & " data=" & CStr(x) & "," & CStr(y)
End Sub
