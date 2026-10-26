' INV: vbal.5.6.12
' SOURCE: spec ms-vbal 5.6.12 member access on Object or Variant is an unbound member resolved at run time
' EXPECT: 7
' EXPECT: 7
' EXPECT: 8
'#MODULE Box class
Public W As Long
'#MODULE Module1 standard
Public Sub Main()
    Dim b As New Box, o As Object, v As Variant
    b.W = 7
    Set o = b
    Set v = b
    Debug.Print CStr(o.W)
    Debug.Print CStr(v.W)
    o.W = 8
    Debug.Print CStr(b.W)
End Sub
