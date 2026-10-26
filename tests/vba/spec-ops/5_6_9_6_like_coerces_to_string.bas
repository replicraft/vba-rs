' INV: vbal.5.6.9.6, op.like
' SOURCE: spec ms-vbal 5.6.9.6 Like operands are Let-coerced to String
' EXPECT: True
' EXPECT: True
' EXPECT: False
Public Sub Main()
    Debug.Print CStr(123 Like "1#3")
    Debug.Print CStr(45 Like "##")
    Debug.Print CStr(45 Like "###")
End Sub
