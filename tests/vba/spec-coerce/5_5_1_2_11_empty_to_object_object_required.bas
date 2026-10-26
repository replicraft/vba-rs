' INV: vbal.5.5.1.2.11, expr.implicit-conv
' SOURCE: spec ms-vbal 5.5.1.2.11 Let-coercion from Empty, Empty to a class or Object is Object required
' XFAIL: wrong VBA behaviour kept where MS-VBAL text differs: Object = Variant without Set is rejected at analysis (SEM007), not error 424 at run time
' ERROR: 424
Public Sub Main()
    Dim v As Variant
    Dim o As Object
    o = v
    Debug.Print "not reached"
End Sub
