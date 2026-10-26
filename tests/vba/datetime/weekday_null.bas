' INV: func.weekday, lit.null
' SOURCE: spec ms-vbal 6.1.2.4.1.13 Weekday of Null is Null
' EXPECT: True
Public Sub Main()
    Debug.Print IsNull(Weekday(Null))
End Sub
