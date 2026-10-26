' INV: vbal.6.1.2.11.1.28, func.monthname
' SOURCE: spec ms-vbal 6.1.2.11.1.28 MonthName numeric designation, Abbreviate defaults False
' EXPECT: a=January
' EXPECT: b=December
' EXPECT: c=Jan
' EXPECT: d=Dec
' EXPECT: e=March
' EXPECT: t=String
Public Sub Main()
    Debug.Print "a=" & MonthName(1)
    Debug.Print "b=" & MonthName(12)
    Debug.Print "c=" & MonthName(1, True)
    Debug.Print "d=" & MonthName(12, True)
    Debug.Print "e=" & MonthName(3, False)
    Debug.Print "t=" & TypeName(MonthName(1))
End Sub
