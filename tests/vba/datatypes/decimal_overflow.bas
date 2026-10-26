' INV: type.decimal
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/data-type-summary
' ERROR: 6
Public Sub Main()
    Dim d As Variant
    d = CDec("79228162514264337593543950335")
    d = d + 1
End Sub
