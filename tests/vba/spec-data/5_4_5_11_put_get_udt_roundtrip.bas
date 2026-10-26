' INV: vbal.5.4.5.11, stmt.put, stmt.get
' SOURCE: spec ms-vbal 5.4.5.12 Get into a UDT reads each member in declaration order
' EXPECT: 9 x 3.5
' GRANT: file-read
' GRANT: file-write
Private Type T
    a As Integer
    s As String * 1
    d As Double
End Type
Public Sub Main()
    MkDir "/t"
    Dim x As T, y As T
    x.a = 9: x.s = "x": x.d = 3.5
    Open "/t/g.bin" For Binary As #1
    Put #1, , x
    Seek #1, 1
    Get #1, , y
    Close #1
    Debug.Print CStr(y.a) & " " & y.s & " " & CStr(y.d)
End Sub
