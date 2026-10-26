' INV: func.cbool
' SOURCE: spec ms-vbal 5.5.1.2.2 Let-coercion to and from Boolean
' EXPECT: True
' EXPECT: False
Public Sub Main()
    Debug.Print CStr(CBool(5 = 5))
    Debug.Print CStr(CBool(0))
End Sub
