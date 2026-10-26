' INV: vbal.6.1.2.3.1.15, func.error
' SOURCE: spec ms-vbal 6.1.2.3.1.15 Error without argument uses most recently raised error
' XFAIL: wrong not implemented: Error$ with no argument and the 0..65535 range
' EXPECT: before=[]
' EXPECT: after=Type mismatch
' EXPECT: afterDollar=Type mismatch
Public Sub Main()
    Debug.Print "before=[" & Error$ & "]"
    On Error Resume Next
    Err.Raise 13
    Debug.Print "after=" & Error
    Debug.Print "afterDollar=" & Error$
End Sub
