' INV: vbal.3.3.2, lit.float
' SOURCE: spec ms-vbal 3.3.2 FLOAT tokens, literal greater than the greatest Single value is invalid
' ERROR: load
Public Sub Main()
    Dim c As Variant
    c = 1.5E39!
End Sub
