' INV: type.decimal
' SOURCE: spec ms-vbal 5.6.9.3.2 plus operator (Decimal)
' EXPECT: True
' EXPECT:  0.3 
Public Sub Main()
    Dim d As Variant
    d = CDec("0.1") + CDec("0.2")
    Debug.Print d = CDec("0.3")
    Debug.Print d
End Sub
