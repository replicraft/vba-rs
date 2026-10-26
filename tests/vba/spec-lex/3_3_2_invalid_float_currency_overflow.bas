' INV: vbal.3.3.2, lit.float
' SOURCE: spec ms-vbal 3.3.2 FLOAT tokens, literal greater than the greatest Currency value is invalid
' ERROR: load
Public Sub Main()
    Dim c As Variant
    c = 922337203685478@
End Sub
