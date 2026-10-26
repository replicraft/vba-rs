' INV: func.dateadd
' SOURCE: spec ms-vbal 6.1.2.4.1.1 DateAdd result before year 100 raises error 5
' ERROR: 5
Public Sub Main()
    Debug.Print DateAdd("yyyy", -2000, DateSerial(1000, 1, 1))
End Sub
