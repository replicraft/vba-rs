' INV: type.variant, lit.empty
' SOURCE: spec ms-vbal 5.6.9.3 arithmetic operators (Empty operand)
' EXPECT:  1 
' EXPECT: []
' EXPECT: TrueTrue
Public Sub Main()
    Dim v As Variant
    Debug.Print v + 1
    Debug.Print "[" & v & "]"
    Debug.Print v = 0; v = ""
End Sub
