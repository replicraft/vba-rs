' INV: vbal.5.5.1.2.10, expr.implicit-conv
' SOURCE: spec ms-vbal 5.5.1.2.10 Let-coercion from Null, Null to Date is Invalid use of Null
' ERROR: 94
Public Sub Main()
    Dim v As Variant
    Dim d As Date
    v = Null
    d = v
    Debug.Print "not reached"
End Sub
