' INV: vbal.5.5.1.2.10, expr.implicit-conv
' SOURCE: spec ms-vbal 5.5.1.2.10 Let-coercion from Null, Null to String is Invalid use of Null
' ERROR: 94
Public Sub Main()
    Dim v As Variant
    Dim s As String
    v = Null
    s = v
    Debug.Print "not reached"
End Sub
