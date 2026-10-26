' INV: vbal.5.2.1.4
' SOURCE: spec ms-vbal 5.2.1.4 Option Private Module in procedural module
' EXPECT: v=7
Option Private Module
Public Sub Main()
    Debug.Print "v=" & CStr(Other.Seven())
End Sub
'#MODULE Other standard
Option Private Module
Public Function Seven() As Long
    Seven = 7
End Function
