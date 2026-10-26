' INV: type.double
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/data-type-summary
' EXPECT:  0.333333333333333 
' EXPECT: False
' ERROR: 6
Public Sub Main()
    Dim d As Double
    d = 1 / 3
    Debug.Print d
    Debug.Print 0.1 + 0.2 = 0.3
    Debug.Print 1E308 * 10
End Sub
