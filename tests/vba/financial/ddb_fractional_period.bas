' INV: func.ddb, vbal.6.1.2.6.1.1
' SOURCE: derived the double-declining balance of period p is the book value at p-1 minus the book value at p, with book(t) = max(salvage, cost * (1 - factor / life) ^ t); for whole periods this is the documented DDB. The fractional extension is unverified against Office.
' EXPECT: 343.46
' EXPECT: 253.37
' EXPECT: 60.14
' EXPECT: 480
' EXPECT: 5
Public Sub Main()
    Debug.Print CStr(Round(DDB(2400, 300, 10, 2.5), 2))
    Debug.Print CStr(Round(DDB(2400, 300, 10, 0.5), 2))
    Debug.Print CStr(Round(DDB(2400, 300, 10, 9.5), 2))
    Debug.Print CStr(DDB(2400, 300, 10, 1))
    On Error Resume Next
    Dim v As Double
    v = DDB(2400, 300, 10, 10.5)
    Debug.Print CStr(Err.Number)
End Sub
