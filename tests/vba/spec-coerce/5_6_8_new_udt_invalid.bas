' INV: vbal.5.6.8, expr.new
' SOURCE: spec ms-vbal 5.6.8 New expressions, New is invalid when the referenced type (a UDT) is not instantiable
' ERROR: load
Type P
    X As Long
End Type
Public Sub Main()
    Dim v As Variant
    Set v = New P
End Sub
