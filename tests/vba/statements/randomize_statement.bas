' INV: stmt.randomize, func.rnd
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/randomize-statement
' EXPECT: True
Public Sub Main()
    Randomize
    Dim x As Double
    x = Rnd
    Debug.Print x >= 0 And x < 1
End Sub
