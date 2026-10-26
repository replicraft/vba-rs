' INV: func.datediff
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/datediff-function
' EXPECT:  2  2  3 
Public Sub Main()
    Debug.Print DateDiff("m", DateSerial(2000, 1, 31), DateSerial(2000, 3, 1)); DateDiff("yyyy", DateSerial(2000, 12, 31), DateSerial(2002, 1, 1)); DateDiff("q", DateSerial(2000, 1, 1), DateSerial(2000, 12, 31))
End Sub
