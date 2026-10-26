' INV: func.dateadd
' SOURCE: spec ms-vbal 6.1.2.4.1.1 DateAdd unknown interval raises error 5
' ERROR: 5
Public Sub Main()
    Debug.Print DateAdd("zz", 1, Date)
End Sub
