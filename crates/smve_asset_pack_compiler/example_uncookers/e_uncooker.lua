SOURCE_EXTENSIONS = { "txt" }
TARGET_EXTENSION = "e"
DEFAULT_CONFIG = { ["character"] = "e" }

function Uncook(buffer, _extension, options)
	for i = 1, #buffer do
		buffer[i] = string.byte(options.character) -- The byte value of the first character
	end

	print("finished uncooking! changed all characters to " .. string.sub(options.character, 1, 1))

	return buffer
end
