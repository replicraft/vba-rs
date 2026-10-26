' INV: vbal.5.4.2.4
' SOURCE: spec ms-vbal 5.4.2.4 explicit-for-each-statement closed by a Next list
' EXPECT: 1a 1b 2a 2b
Public Sub Main()
    Dim n As Variant, l As Variant, s As String
    Dim nums(0 To 1) As Variant, lets(0 To 1) As Variant
    nums(0) = 1: nums(1) = 2
    lets(0) = "a": lets(1) = "b"
    For Each n In nums
        For Each l In lets
            s = s & CStr(n) & CStr(l) & " "
    Next l, n
    Debug.Print Trim$(s)
End Sub
