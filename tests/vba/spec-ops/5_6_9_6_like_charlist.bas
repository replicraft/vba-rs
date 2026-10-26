' INV: vbal.5.6.9.6, op.like
' SOURCE: spec ms-vbal 5.6.9.6 Like character lists, ranges, negation, leading/trailing dash
' EXPECT: TFTFTFTFTFTFTFFT
Private Function B(ByVal x As Boolean) As String
    If x Then B = "T" Else B = "F"
End Function
Public Sub Main()
    Dim r As String
    r = B("b" Like "[abc]") & B("d" Like "[abc]")
    r = r & B("b" Like "[a-c]") & B("d" Like "[a-c]")
    r = r & B("d" Like "[!abc]") & B("b" Like "[!abc]")
    r = r & B("d" Like "[!a-c]") & B("c" Like "[!a-c]")
    r = r & B("-" Like "[-a]") & B("b" Like "[-a]")
    r = r & B("-" Like "[a-]") & B("b" Like "[a-]")
    r = r & B("x" Like "[a-cx-z]") & B("m" Like "[a-cx-z]")
    r = r & B("-" Like "[!-a]") & B("b" Like "[!-a]")
    Debug.Print r
End Sub
