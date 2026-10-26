' INV: vbal.5.2.3.4
' SOURCE: spec ms-vbal 5.2.3.4 enum declared in a class module
' EXPECT: 3
Public Sub Main()
    Dim c As New Holder
    Debug.Print CStr(c.Pick())
End Sub
'#MODULE Holder class
Private Enum Local
    One = 1
    Two
    Three
End Enum
Public Function Pick() As Long
    Pick = Three
End Function
