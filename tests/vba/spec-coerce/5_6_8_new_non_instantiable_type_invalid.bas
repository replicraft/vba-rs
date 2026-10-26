' INV: vbal.5.6.8, expr.new
' SOURCE: spec ms-vbal 5.6.8 New expressions, New is invalid when the referenced type is not instantiable
' ERROR: load
Public Sub Main()
    Dim v As Variant
    Set v = New Long
End Sub
