' INV: expr.paren
' SOURCE: spec ms-vbal 5.6.6 parenthesized expressions (with 5.6.9.1 precedence, 5.4.5.8 Print numeric output)
' EXPECT:  9  7 
Public Sub Main()
    Debug.Print (1 + 2) * 3; 1 + (2 * 3)
End Sub
