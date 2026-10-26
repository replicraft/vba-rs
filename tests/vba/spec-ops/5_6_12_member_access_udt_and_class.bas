' INV: vbal.5.6.12
' SOURCE: spec ms-vbal 5.6.12 member access on UDT and specific class declared types
' EXPECT: 3
' EXPECT: 8
' EXPECT: hi
' EXPECT: 15
'#MODULE Box class
Public W As Long
Public Function Area(ByVal h As Long) As Long
    Area = W * h
End Function
Public Sub Greet()
    Debug.Print "hi"
End Sub
'#MODULE Module1 standard
Private Type P
    X As Long
    Y As Long
End Type
Public Sub Main()
    Dim u As P, b As New Box
    u.X = 3
    u.Y = 8
    Debug.Print CStr(u.X)
    Debug.Print CStr(u.Y)
    b.Greet
    b.W = 5
    Debug.Print CStr(b.Area(3))
End Sub
