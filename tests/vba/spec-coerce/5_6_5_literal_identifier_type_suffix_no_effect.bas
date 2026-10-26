' INV: vbal.5.6.5
' SOURCE: spec ms-vbal 5.6.5 Literal expressions, a type suffix after a literal identifier has no effect
' XFAIL: wrong not implemented: type suffix on True/False/Empty/Null
' EXPECT: Boolean
' EXPECT: Boolean
' EXPECT: Empty
' EXPECT: Null
Public Sub Main()
    Debug.Print TypeName(True%)
    Debug.Print TypeName(False&)
    Debug.Print TypeName(Empty$)
    Debug.Print TypeName(Null#)
End Sub
