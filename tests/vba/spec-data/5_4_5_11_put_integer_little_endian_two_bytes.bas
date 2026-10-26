' INV: vbal.5.4.5.11, stmt.put
' SOURCE: spec ms-vbal 5.4.5.11 Integer is written as two bytes, little endian
' EXPECT: len=2 b1=2 b2=1
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Dim i As Integer, x As Byte, y As Byte
    i = 258
    Open "/t/g.bin" For Binary As #1
    Put #1, , i
    Seek #1, 1
    Get #1, , x
    Get #1, , y
    Debug.Print "len=" & CStr(LOF(1)) & " b1=" & CStr(x) & " b2=" & CStr(y)
    Close #1
End Sub
