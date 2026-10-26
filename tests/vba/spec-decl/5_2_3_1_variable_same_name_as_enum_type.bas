' INV: vbal.5.2.3.1
' SOURCE: spec ms-vbal 5.2.3.1 variable may share the enum name of an enum in the same module
' XFAIL: wrong pending
' EXPECT: ok=4
Private Enum Color
    Red = 1
End Enum
Private Color As Long
Public Sub Main()
    Color = 4
    Debug.Print "ok=" & CStr(Color)
End Sub
