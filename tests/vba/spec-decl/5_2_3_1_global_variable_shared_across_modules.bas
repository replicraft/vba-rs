' INV: vbal.5.2.3.1
' SOURCE: spec ms-vbal 5.2.3.1 Global and Public variables are public with module extent
' EXPECT: g=11 p=22
' EXPECT: again=12
Global g As Long
Public Sub Main()
    g = 11
    Other.p = 22
    Debug.Print "g=" & CStr(g) & " p=" & CStr(Other.p)
    Other.Bump
    Debug.Print "again=" & CStr(Other.G2())
End Sub
'#MODULE Other standard
Public p As Long
Public Sub Bump()
    g = g + 1
End Sub
Public Function G2() As Long
    G2 = g
End Function
