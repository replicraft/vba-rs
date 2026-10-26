' INV: vbal.6.1.2.4.1.5, func.datevalue
' SOURCE: spec ms-vbal 6.1.2.4.1.5 DateValue raises an error when the string has invalid time information
' ERROR: 13
Public Sub Main()
    Dim v As Variant
    v = DateValue("12/30/1991 89:98")
    Debug.Print "reached"
End Sub
