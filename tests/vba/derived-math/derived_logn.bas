' INV: derived.logn, func.log
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/derived-math-functions
' EXPECT: 10
' EXPECT: 3
Function LogN(X As Double, N As Double) As Double
    LogN = Log(X) / Log(N)
End Function
Public Sub Main()
    Debug.Print CStr(Round(LogN(1024, 2), 6))
    Debug.Print CStr(Round(LogN(1000, 10), 6))
End Sub
