' INV: stmt.function
' SOURCE: derived a Function declared As String() returns an array that the caller can store and index
' EXPECT:  1  3 
' EXPECT: a-c
Function Parts(ByVal s As String) As String()
    Parts = Split(s, ",")
End Function
Public Sub Main()
    Dim p() As String
    p = Parts("a,b,c")
    Debug.Print LBound(p) + 1; UBound(p) + 1
    Debug.Print p(0) & "-" & Parts("a,b,c")(2)
End Sub
