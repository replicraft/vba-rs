' INV: vbal.5.5.1.2.13, expr.implicit-conv
' SOURCE: spec ms-vbal 5.5.1.2.13 Let-coercion to and from a class, an object without a default member has no simple data value: error 438
' ERROR: 438
'#MODULE Plain class
Public Sub Hello()
End Sub
'#MODULE Module1 standard
Public Sub Main()
    Dim v As Variant
    Dim n As Long
    Set v = New Plain
    n = v
    Debug.Print "not reached"
End Sub
