' INV: func.log
' SOURCE: derived Log of a non-positive number is an invalid-procedure-call error 5
' ERROR: 5
Public Sub Main()
    Debug.Print Log(0)
End Sub
