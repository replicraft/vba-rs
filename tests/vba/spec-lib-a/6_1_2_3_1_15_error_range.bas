' INV: vbal.6.1.2.3.1.15, func.error
' SOURCE: spec ms-vbal 6.1.2.3.1.15 Error/Error$ argument range
' XFAIL: wrong VBA behaviour kept where MS-VBAL text differs: an argument outside 0..65535 is error 5 (the expectation of error 6 for 65536 and of success for negatives is not confirmed against real VBA)
' EXPECT: over=6
' EXPECT: overDollar=6
' EXPECT: max=0
' EXPECT: neg=0
' EXPECT: text=13
Public Sub Main()
    Dim v As Variant
    On Error Resume Next
    Err.Clear: v = Error(65536): Debug.Print "over=" & Err.Number
    Err.Clear: v = Error$(65536): Debug.Print "overDollar=" & Err.Number
    Err.Clear: v = Error$(65535): Debug.Print "max=" & Err.Number
    Err.Clear: v = Error$(-1): Debug.Print "neg=" & Err.Number
    Err.Clear: v = Error$("abc"): Debug.Print "text=" & Err.Number
End Sub
