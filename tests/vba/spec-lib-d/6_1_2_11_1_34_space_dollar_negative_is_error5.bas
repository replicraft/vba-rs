' INV: vbal.6.1.2.11.1.34, func.space
' SOURCE: derived ms-vbal 6.1.2.11.1.34 Space$ returns spaces like Space; a negative count is error 5
' EXPECT: a=3
' EXPECT: b=0
' EXPECT: err=5
Public Sub Main()
    Debug.Print "a=" & CStr(Len(Space$(3)))
    Debug.Print "b=" & CStr(Len(Space$(0)))
    On Error Resume Next
    Dim s As String
    s = Space$(-1)
    Debug.Print "err=" & CStr(Err.Number)
End Sub
