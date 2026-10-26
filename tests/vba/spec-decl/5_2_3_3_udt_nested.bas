' INV: vbal.5.2.3.3
' SOURCE: spec ms-vbal 5.2.3.3 UDT member whose type is another UDT
' EXPECT: 3 4
Private Type Pt
    x As Long
    y As Long
End Type
Private Type Seg
    a As Pt
    b As Pt
End Type
Public Sub Main()
    Dim s As Seg
    s.a.x = 3
    s.b.y = 4
    Debug.Print CStr(s.a.x) & " " & CStr(s.b.y)
End Sub
