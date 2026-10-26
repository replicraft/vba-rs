' INV: vbal.5.6.9.5, op.eq, op.lt
' SOURCE: spec ms-vbal 5.6.9.5 relational Date effective type
' EXPECT: True
' EXPECT: True
' EXPECT: True
' EXPECT: True
Public Sub Main()
    Dim d As Date
    d = #1/2/2020#
    Debug.Print CStr(d = "1/2/2020")
    Debug.Print CStr(d > 0)
    Debug.Print CStr(#12/30/1899# = 0)
    Debug.Print CStr(#1/1/2020# < d)
End Sub
