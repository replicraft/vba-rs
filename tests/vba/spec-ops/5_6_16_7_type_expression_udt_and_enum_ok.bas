' INV: vbal.5.6.16.7
' SOURCE: spec ms-vbal 5.6.16.7 defined type expressions bind in type context to UDT, Enum and class types
' EXPECT: 1 2 5
'#MODULE Cls class
Public V As Long
'#MODULE Module1 standard
Private Type Q
    Y As Long
End Type
Private Enum E
    One = 1
End Enum
Public Sub Main()
    Dim u As Q, c As New Cls
    Dim m As E
    m = One
    u.Y = 2
    c.V = 5
    Debug.Print CStr(m) & " " & CStr(u.Y) & " " & CStr(c.V)
End Sub
