' INV: vbal.5.4.5.11, stmt.put
' SOURCE: spec ms-vbal 5.4.5.11 Boolean True is FF FF, False is 00 00
' EXPECT: 255 255 0 0
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Dim t As Boolean, f As Boolean
    Dim a As Byte, b As Byte, c As Byte, d As Byte
    t = True: f = False
    Open "/t/g.bin" For Binary As #1
    Put #1, , t
    Put #1, , f
    Seek #1, 1
    Get #1, , a
    Get #1, , b
    Get #1, , c
    Get #1, , d
    Close #1
    Debug.Print CStr(a) & " " & CStr(b) & " " & CStr(c) & " " & CStr(d)
End Sub
