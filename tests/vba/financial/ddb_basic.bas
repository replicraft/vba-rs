' INV: func.ddb
' SOURCE: derived closed-form annuity formula from the function's documented definition; value rounded to 2 places
' EXPECT: 480
' EXPECT: 384
' EXPECT: 307.2
Public Sub Main()
    Debug.Print CStr(DDB(2400, 300, 10, 1))
    Debug.Print CStr(DDB(2400, 300, 10, 2))
    Debug.Print CStr(Round(DDB(2400, 300, 10, 3), 2))
End Sub
