' INV: type.decimal
' SOURCE: spec ms-vbal 5.6.9.3 arithmetic overflow (Variant, Decimal)
' ERROR: 6
Public Sub Main()
    Dim d As Variant
    d = CDec("79228162514264337593543950335")
    d = d + 1
End Sub
