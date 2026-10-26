' INV: vbal.5.5.1.2.10, expr.implicit-conv
' SOURCE: spec ms-vbal 5.5.1.2.10 Let-coercion from Null, Null to String * length is Invalid use of Null
' ERROR: 94
Public Sub Main()
    Dim v As Variant
    Dim f As String * 3
    v = Null
    f = v
    Debug.Print "not reached"
End Sub
