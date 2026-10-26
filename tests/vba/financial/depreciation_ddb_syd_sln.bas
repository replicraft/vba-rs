' INV: func.ddb
' SOURCE: derived the Learn DDB, SYD and SLN pages: double declining balance takes 2/life of the remaining book value, sum of years digits weights by remaining life; the values were evaluated outside the interpreter
' EXPECT: 222.22
' EXPECT: 172.84
' EXPECT: 480.00
' EXPECT: 180.00
' EXPECT: 20.00
' EXPECT: 4090.91
' EXPECT: 2250.00
Public Sub Main()
    Debug.Print Format(DDB(1000, 100, 9, 1), "0.00")
    Debug.Print Format(DDB(1000, 100, 9, 2), "0.00")
    Debug.Print Format(DDB(2400, 300, 10, 1), "0.00")
    Debug.Print Format(SYD(1000, 100, 9, 1), "0.00")
    Debug.Print Format(SYD(1000, 100, 9, 9), "0.00")
    Debug.Print Format(SYD(30000, 7500, 10, 1), "0.00")
    Debug.Print Format(SLN(30000, 7500, 10), "0.00")
End Sub
