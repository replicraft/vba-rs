' INV: type.decimal
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/data-type-summary
' EXPECT:  79228162514264337593543950335 
' EXPECT:  0.0000000000000000000000000001 
Public Sub Main()
    Dim d As Variant
    d = CDec("79228162514264337593543950335")
    Debug.Print d
    d = CDec("0.0000000000000000000000000001")
    Debug.Print d
End Sub
